use crate::ast::{
    self, BinaryExpressionOperator, BinaryExpressionType, CreateValueNodeOptions, DynamicNode,
};
use crate::variable::VariableType;
use fimpp_lexer::lexer;
use fimpp_lexer::token::TokenType;

#[test]
fn binary_expression_add() {
    let source = "1 plus 1";
    let tokens = lexer::parse(source);
    // ignore EOF
    let tokens = &tokens[..tokens.len() - 1];
    let node = ast::create_value_node(
        tokens,
        CreateValueNodeOptions {
            possible_null_type: None,
            into_array: false,
        },
    )
    .expect("create_value_node failed");

    match node {
        DynamicNode::BinaryExpression(b) => {
            assert_eq!(b.binary_type, BinaryExpressionType::Arithmetic);
            assert_eq!(b.operator, BinaryExpressionOperator::Add);
            match &b.left {
                DynamicNode::Literal(_) => {}
                _ => panic!("left not literal"),
            }
            match &b.right {
                DynamicNode::Literal(_) => {}
                _ => panic!("right not literal"),
            }
        }
        _ => panic!("expected binary expression"),
    }
}

#[test]
fn binary_expression_multiple() {
    let source = "1 plus 2 minus 3";
    let tokens = lexer::parse(source);
    let tokens = &tokens[..tokens.len() - 1];
    let node = ast::create_value_node(
        tokens,
        CreateValueNodeOptions {
            possible_null_type: None,
            into_array: false,
        },
    )
    .expect("create_value_node failed");

    match node {
        DynamicNode::BinaryExpression(b) => {
            assert_eq!(b.binary_type, BinaryExpressionType::Arithmetic);
            assert_eq!(b.operator, BinaryExpressionOperator::Sub);
            let left_ref = match &b.left {
                DynamicNode::BinaryExpression(bb) => bb,
                _ => panic!("left not binary"),
            };
            assert_eq!(left_ref.binary_type, BinaryExpressionType::Arithmetic);
            assert_eq!(left_ref.operator, BinaryExpressionOperator::Add);
            match &left_ref.left {
                DynamicNode::Literal(_) => {}
                _ => panic!("left.left not literal"),
            }
            match &left_ref.right {
                DynamicNode::Literal(l) => {
                    assert_eq!(l.variable.get_value_number(), 2.0);
                }
                _ => panic!("left.right not literal"),
            }
            match &b.right {
                DynamicNode::Literal(r) => {
                    assert_eq!(r.variable.get_value_number(), 3.0);
                }
                _ => panic!("right not literal"),
            }
        }
        _ => panic!("expected binary expression"),
    }
}

#[test]
fn binary_expression_relational() {
    let source = "correct is equal to true";
    let tokens = lexer::parse(source);
    let tokens = &tokens[..tokens.len() - 1];
    let node = ast::create_value_node(
        tokens,
        CreateValueNodeOptions {
            possible_null_type: None,
            into_array: false,
        },
    )
    .expect("create_value_node failed");

    match node {
        DynamicNode::BinaryExpression(b) => {
            assert_eq!(b.binary_type, BinaryExpressionType::Relational);
            assert_eq!(b.operator, BinaryExpressionOperator::Eq);
            match &b.left {
                DynamicNode::Literal(_) => {}
                _ => panic!("left not literal"),
            }
            match &b.right {
                DynamicNode::Literal(_) => {}
                _ => panic!("right not literal"),
            }
        }
        _ => panic!("expected binary expression"),
    }
}
