import assert from 'node:assert/strict';
import { mkdirSync, mkdtempSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { spawnSync } from 'node:child_process';
import { API } from 'typescript/unstable/sync';
import { SyntaxKind as K } from 'typescript/unstable/ast';

const binary=process.env.UTSC_BIN ?? resolve('target/debug/utsc');
if (!process.env.UTSC_BIN) {
  const build=spawnSync('cargo',['build','--locked'],{stdio:'inherit'});
  if(build.error) throw build.error;
  assert.equal(build.status,0);
}
const names=new Map();
for(const [name,value] of Object.entries(K)) if(typeof value==='number'&&!names.has(value)) names.set(value,name);
const sources=[
  'a + b * c ** d ** e','a = b += c','a >>> b + c >= d','a, b = c','a >>= b','a >>>= b','a >= b','a >> b',
  '(a + b) * c','-a * b','a ** -b','(-a) ** b','++a ** b','(a) = b',
  'a ? b = c : d ? e : f','a ?? (b || c)','(a && b) ?? c',
  'f(a, b ? c : d, ...rest,)','f()[0].x','obj?.fn?.(x)?.[i].default',
  '[,a,,...xs,]','this.foo + super.bar','typeof a === "string"','delete obj[key]',
  'void a + ~b','x++ + --y','`hello`','`😀${a ? `inner${b}` : c}tail${fn?.(x)?.[i]}end`',
  '变量 + café * 𐐀name','"😀" + "𐐀"','\\u0061 + b','a /*comment*/ + b',
];
let seed=0x7a11;
const rand=n=>{seed^=seed<<13;seed^=seed>>>17;seed^=seed<<5;return (seed>>>0)%n;};
const atoms=['a','b','变量','café','𐐀name','42','1.5','0xff','123n',"'😀'",'true','null','this'];
const operators=['+','-','*','/','%','**','<','<=','>','>=','>>','>>>','<<','==','===','!=','!==','&','|','^','&&','||','??'];
function expression(depth) {
  if(!depth||rand(4)===0) return atoms[rand(atoms.length)];
  const child=()=>expression(depth-1);
  switch(rand(8)) {
    case 0:return `(${child()}) ${operators[rand(operators.length)]} (${child()})`;
    case 1:return `(${child()}) ? (${child()}) : (${child()})`;
    case 2:return `[${child()},,${child()},...xs]`;
    case 3:return `fn(${child()},...xs,${child()})`;
    case 4:return `(${child()})?.field?.[${child()}]`;
    case 5:return `!(${child()})`;
    case 6:return '`head${'+child()+'}中${'+child()+'}tail`';
    default:return `(${child()}) + (${child()}) * (${child()})`;
  }
}
for(let i=0;i<1500;i++) sources.push(expression(1+rand(4)));
const prefix=i=>`const expression${i} = (`;
let text='';
const offsets=[];
for(let i=0;i<sources.length;i++) {
  text+=prefix(i);offsets.push(text.length);text+=sources[i]+');\n';
}
mkdirSync('target',{recursive:true});
const root=mkdtempSync(resolve('target/parser-compat-'));
writeFileSync(root+'/tsconfig.json',JSON.stringify({compilerOptions:{noLib:true,noEmit:true,target:'ESNext'},files:['index.ts']}));
writeFileSync(root+'/index.ts',text);
const requests=sources.map((source,i)=>JSON.stringify({name:String(i),source})).join('\n')+'\n';
const native=spawnSync(binary,['parse-expression','--jsonl'],{input:requests,encoding:'utf8',timeout:60_000,maxBuffer:128*1024*1024});
if(native.error) throw native.error;
assert.ok(native.status===0||native.status===1,`Native parser failed: ${native.stderr}`);
const responses=native.stdout.trimEnd().split('\n').map(line=>JSON.parse(line));
assert.equal(responses.length,sources.length);
const api=new API({cwd:root});
let failures=0,nodeCount=0;
try {
  const snapshot=api.updateSnapshot({openProjects:[root+'/tsconfig.json']});
  const project=snapshot.getProject(root+'/tsconfig.json');
  const file=project.program.getSourceFile(root+'/index.ts');
  const syntax=project.program.getSyntacticDiagnostics();
  assert.equal(syntax.length,0,`Reference fixture has syntactic diagnostics: ${JSON.stringify(syntax).slice(0,1000)}`);
  assert.equal(file.statements.length,sources.length);
  for(let i=0;i<sources.length;i++) {
    const offset=offsets[i], response=responses[i];
    const span=node=>[node.getStart(file)-offset,node.end-offset];
    const token=node=>({span:span(node),text:node.getText(file)});
    const reference=node=>{
      const base={span:span(node)};
      switch(node.kind) {
        case K.Identifier:case K.ThisKeyword:case K.SuperKeyword:return {...base,kind:'Identifier',text:node.getText(file)};
        case K.NumericLiteral:case K.BigIntLiteral:case K.StringLiteral:case K.NoSubstitutionTemplateLiteral:case K.TrueKeyword:case K.FalseKeyword:case K.NullKeyword:return {...base,kind:'Literal',text:node.getText(file)};
        case K.ParenthesizedExpression:return {...base,kind:'Parenthesized',expression:reference(node.expression)};
        case K.BinaryExpression:return {...base,kind:'Binary',operator:token(node.operatorToken),left:reference(node.left),right:reference(node.right)};
        case K.PrefixUnaryExpression:case K.PostfixUnaryExpression:return {...base,kind:'Unary',operator:names.get(node.operator),postfix:node.kind===K.PostfixUnaryExpression,operand:reference(node.operand)};
        case K.TypeOfExpression:case K.VoidExpression:case K.DeleteExpression:return {...base,kind:'Unary',operator:node.kind===K.TypeOfExpression?'TypeOfKeyword':node.kind===K.VoidExpression?'VoidKeyword':'DeleteKeyword',postfix:false,operand:reference(node.expression)};
        case K.ConditionalExpression:return {...base,kind:'Conditional',condition:reference(node.condition),when_true:reference(node.whenTrue),when_false:reference(node.whenFalse)};
        case K.CallExpression:return {...base,kind:'Call',callee:reference(node.expression),optional:!!node.questionDotToken,arguments:node.arguments.map(reference)};
        case K.PropertyAccessExpression:return {...base,kind:'Property',object:reference(node.expression),optional:!!node.questionDotToken,name:token(node.name)};
        case K.ElementAccessExpression:return {...base,kind:'Element',object:reference(node.expression),optional:!!node.questionDotToken,index:reference(node.argumentExpression)};
        case K.ArrayLiteralExpression:return {...base,kind:'Array',elements:node.elements.map(element=>element.kind===K.OmittedExpression?null:reference(element))};
        case K.SpreadElement:return {...base,kind:'Spread',expression:reference(node.expression)};
        case K.TemplateExpression:return {...base,kind:'Template',parts:[token(node.head),...node.templateSpans.map(part=>token(part.literal))],expressions:node.templateSpans.map(part=>reference(part.expression))};
        default:throw new Error(`Unhandled reference node ${names.get(node.kind)}`);
      }
    };
    const actual=id=>{
      const node=response.nodes[id];
      assert.ok(node,`Invalid arena node ${id}`);
      nodeCount++;
      const {start,end}=node.span,base={span:[start,end],kind:node.kind};
      const t=tok=>({span:[tok.start,tok.end],text:tok.text});
      switch(node.kind) {
        case 'Identifier':case 'Literal':return {...base,text:node.token.text};
        case 'Parenthesized':case 'Spread':return {...base,expression:actual(node.expression)};
        case 'Binary':return {...base,operator:t(node.operator),left:actual(node.left),right:actual(node.right)};
        case 'Unary':return {...base,operator:node.operator.kind,postfix:node.postfix,operand:actual(node.operand)};
        case 'Conditional':return {...base,condition:actual(node.condition),when_true:actual(node.when_true),when_false:actual(node.when_false)};
        case 'Call':return {...base,callee:actual(node.callee),optional:node.optional,arguments:node.arguments.map(actual)};
        case 'Property':return {...base,object:actual(node.object),optional:node.optional,name:t(node.name)};
        case 'Element':return {...base,object:actual(node.object),optional:node.optional,index:actual(node.index)};
        case 'Array':return {...base,elements:node.elements.map(id=>id===null?null:actual(id))};
        case 'Template':return {...base,parts:node.parts.map(t),expressions:node.expressions.map(actual)};
        default:throw new Error(`Unhandled native node ${node.kind}`);
      }
    };
    try {
      assert.deepEqual(response.diagnostics,[]);
      assert.deepEqual(response.lexical_diagnostics,[]);
      assert.notEqual(response.root,null);
      assert.deepEqual(actual(response.root),reference(file.statements[i].declarationList.declarations[0].initializer.expression));
    } catch(error) {
      failures++;
      if(failures<=5) console.error(`FAIL ${i}: ${sources[i].slice(0,250)}\n${error.message.slice(0,2500)}`);
    }
  }
  snapshot.dispose();
} finally {api.close();}
console.log(`TypeScript 7.0.2 native parser: ${sources.length-failures}/${sources.length} expression trees match; ${nodeCount} nodes visited. Compared typed structure, operators, raw spelling, and UTF-16 spans.`);
console.log('This is expression-subset coverage, not source-file/checker/emit conformance.');
if(failures) process.exitCode=1;
